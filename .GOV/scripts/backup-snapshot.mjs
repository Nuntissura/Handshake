#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  REPO_ROOT,
  WORKSPACE_ROOT,
  PROTECTED_BRANCHES,
  absFromRepo,
  buildDynamicTopologySnapshot,
  buildTopologyRegistry,
  discoverGitCheckouts,
  ensureDir,
  parseOriginRepo,
  refExists,
  relFromWorkspace,
  resolveBackupRoot,
  resolveNasBackupRoot,
  runGit,
  runRobocopy,
  timestampForSnapshot,
  writeFileNormalized,
} from "./git-topology-lib.mjs";

const OFFLINE_GIT_BACKUP_SETUP_REPO_PATH = ".GOV/roles_shared/OFFLINE_GIT_BACKUP_SETUP.md";

function usage() {
  console.error("Usage: node .GOV/scripts/backup-snapshot.mjs [--label <name>] [--out-root <dir>] [--nas-root <dir>] [--require-nas]");
  process.exit(1);
}

function parseArgs() {
  const args = process.argv.slice(2);
  let label = "manual";
  let outRoot = "";
  let nasRoot = "";
  let requireNas = false;
  for (let i = 0; i < args.length; i += 1) {
    const token = args[i];
    if (token === "--label") {
      const value = (args[i + 1] || "").trim();
      if (value && !value.startsWith("--")) {
        label = value;
        i += 1;
      }
      continue;
    }
    if (token === "--out-root") {
      const value = (args[i + 1] || "").trim();
      if (value && !value.startsWith("--")) {
        outRoot = value;
        i += 1;
      }
      continue;
    }
    if (token === "--nas-root") {
      const value = (args[i + 1] || "").trim();
      if (value && !value.startsWith("--")) {
        nasRoot = value;
        i += 1;
      }
      continue;
    }
    if (token === "--require-nas") {
      requireNas = true;
      continue;
    }
    usage();
  }
  return { label: label || "manual", outRoot, nasRoot, requireNas };
}

function createBundle(absPath, refs) {
  execFileSync("git", ["bundle", "create", absPath, ...refs], { cwd: REPO_ROOT, stdio: "inherit" });
}

function writeReusableGuide(destRoot) {
  if (!destRoot) return;
  const sourceText = fs.readFileSync(absFromRepo(OFFLINE_GIT_BACKUP_SETUP_REPO_PATH), "utf8");
  writeFileNormalized(path.join(destRoot, "OFFLINE_GIT_BACKUP_SETUP.md"), sourceText);
}

function resolveProtectedBundleRefs() {
  return PROTECTED_BRANCHES.map((branch) => {
    if (refExists(REPO_ROOT, branch)) return branch;
    const remoteRef = `refs/remotes/origin/${branch}`;
    if (refExists(REPO_ROOT, remoteRef)) return remoteRef;
    return "";
  }).filter(Boolean);
}

const { label, outRoot, nasRoot, requireNas } = parseArgs();
const snapshotStamp = timestampForSnapshot();
const backupRoot = resolveBackupRoot(outRoot);
const nasBackupRoot = resolveNasBackupRoot(nasRoot);
if (requireNas && !nasBackupRoot) {
  console.error("HANDSHAKE_NAS_BACKUP_ROOT is not configured");
  process.exit(1);
}
const snapshotName = `${snapshotStamp}-${label.replace(/[^A-Za-z0-9._-]+/g, "-")}`;
const snapshotRoot = path.join(backupRoot, snapshotName);
const bundlesDir = path.join(snapshotRoot, "bundles");
const worktreesDir = path.join(snapshotRoot, "worktrees");
const manifestsDir = path.join(snapshotRoot, "manifests");

ensureDir(bundlesDir);
ensureDir(worktreesDir);
ensureDir(manifestsDir);
writeReusableGuide(backupRoot);

const topologyRegistry = buildTopologyRegistry();
const topologySnapshot = buildDynamicTopologySnapshot();
const gitCheckouts = discoverGitCheckouts();
const originRepo = parseOriginRepo();

createBundle(path.join(bundlesDir, "all_refs.bundle"), ["--all"]);
createBundle(path.join(bundlesDir, "protected_branches.bundle"), resolveProtectedBundleRefs());

const copiedWorktrees = [];
for (const checkout of gitCheckouts) {
  const destDir = path.join(worktreesDir, checkout.id);
  ensureDir(destDir);
  runRobocopy(checkout.abs_dir, destDir);
  copiedWorktrees.push({
    id: checkout.id,
    source_rel_path_from_workspace: relFromWorkspace(checkout.abs_dir),
    dest_rel_path_in_snapshot: path.relative(snapshotRoot, destDir).replace(/\\/g, "/"),
  });
}

const manifest = {
  schema_version: "hsk.repo_resilience_snapshot@0.1",
  created_at_utc: new Date().toISOString(),
  label,
  snapshot_name: snapshotName,
  repo_root: REPO_ROOT,
  workspace_root: WORKSPACE_ROOT,
  origin: originRepo.owner && originRepo.repo ? `${originRepo.owner}/${originRepo.repo}` : runGit(["remote", "get-url", "origin"]),
  backup_root: backupRoot,
  nas_backup_root: nasBackupRoot || "NOT_CONFIGURED",
  bundles: [
    "bundles/all_refs.bundle",
    "bundles/protected_branches.bundle",
  ],
  copied_worktrees: copiedWorktrees,
  topology_registry: topologyRegistry,
  topology_snapshot: topologySnapshot,
  notes: [
    "git bundles preserve committed refs",
    "robocopy worktree copies preserve working files, including dirty state, outside the repo tree",
    "robocopy excludes .git and common build-cache directories to keep snapshots portable",
    "backup roots are append-only timestamped directories; old snapshots are not deleted by the snapshot job",
  ],
};

writeFileNormalized(path.join(manifestsDir, "repo_resilience_manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
writeFileNormalized(path.join(manifestsDir, "git_topology_registry.json"), `${JSON.stringify(topologyRegistry, null, 2)}\n`);
writeFileNormalized(path.join(manifestsDir, "git_topology_snapshot.json"), `${JSON.stringify(topologySnapshot, null, 2)}\n`);
writeFileNormalized(
  path.join(manifestsDir, "restore_instructions.txt"),
  [
    "Restore bundles:",
    "  git clone <repo-url> restored-handshake",
    "  git -C restored-handshake fetch ../bundles/all_refs.bundle \"refs/*:refs/*\"",
    "",
    "Inspect copied working trees under worktrees/ for dirty or uncommitted file recovery.",
    "If NAS backup root is configured, the entire snapshot directory is copied there as a timestamped snapshot.",
    "",
  ].join("\n"),
);

let nasCopyStatus = "SKIPPED";
if (nasBackupRoot) {
  writeReusableGuide(nasBackupRoot);
  const nasDest = path.join(nasBackupRoot, snapshotName);
  ensureDir(nasDest);
  runRobocopy(snapshotRoot, nasDest, []);
  nasCopyStatus = `COPIED_TO_${nasDest}`;
}

console.log(`[BACKUP_SNAPSHOT] snapshot_root=${snapshotRoot}`);
console.log(`[BACKUP_SNAPSHOT] bundles=all_refs.bundle, protected_branches.bundle`);
console.log(`[BACKUP_SNAPSHOT] copied_worktrees=${copiedWorktrees.map((row) => row.id).join(", ")}`);
console.log(`[BACKUP_SNAPSHOT] nas_copy=${nasCopyStatus}`);
