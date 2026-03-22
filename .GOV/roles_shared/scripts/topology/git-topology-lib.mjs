#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { SHARED_GOV_GIT_TOPOLOGY_FILE } from "../lib/runtime-paths.mjs";

export const SCHEMA_VERSION = "hsk.git_topology_registry@0.1";
export const DYNAMIC_SNAPSHOT_SCHEMA_VERSION = "hsk.git_topology_snapshot@0.1";
export const TOPOLOGY_REGISTRY_JSON_PATH = SHARED_GOV_GIT_TOPOLOGY_FILE;
export const TOPOLOGY_REGISTRY_MD_PATH = ".GOV/roles_shared/records/GIT_TOPOLOGY_REGISTRY.md";
export const PROTECTED_BRANCHES = ["main", "user_ilja", "gov_kernel"];
export const WORKTREE_SPECS = [
  {
    id: "handshake_main",
    rel_path: "../handshake_main",
    local_branch: "main",
    remote_branch: "origin/main",
    role: "CANONICAL",
    canonical: true,
    description: "Canonical integrated checkout on disk",
  },
  {
    id: "wt-ilja",
    rel_path: "../wt-ilja",
    local_branch: "user_ilja",
    remote_branch: "origin/user_ilja",
    role: "OPERATOR",
    canonical: false,
    description: "Operator worktree derived from main; remote branch is backup only",
  },
  {
    id: "wt-gov-kernel",
    rel_path: "../wt-gov-kernel",
    local_branch: "gov_kernel",
    remote_branch: "origin/gov_kernel",
    role: "GOV_KERNEL",
    canonical: false,
    description: "Orchestrator governance kernel worktree (canonical .GOV/ source)",
  },
];

export const SNAPSHOT_EXCLUDE_DIRS = [
  ".git",
  "node_modules",
  ".next",
  ".turbo",
  "dist",
  "build",
  "target",
  ".pnpm-store",
];
export const SNAPSHOT_EXCLUDE_FILES = [".git"];

function resolveRepoRoot() {
  try {
    const out = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (out) return out;
  } catch {
    // Ignore and fall back to file-relative resolution.
  }
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
}

export const REPO_ROOT = path.resolve(resolveRepoRoot());
export const WORKSPACE_ROOT = path.resolve(REPO_ROOT, "..");

export const compareStrings = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
export const toPosix = (value) => String(value || "").replace(/\\/g, "/");
export const absFromRepo = (relPath) => path.resolve(REPO_ROOT, relPath);
export const relFromRepo = (absPath) => toPosix(path.relative(REPO_ROOT, absPath));
export const absFromWorkspace = (relPath) => path.resolve(WORKSPACE_ROOT, relPath);
export const relFromWorkspace = (absPath) => toPosix(path.relative(WORKSPACE_ROOT, absPath));

export function runGit(args, options = {}) {
  return execFileSync("git", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
    ...options,
  }).trim();
}

export function runGitInRepo(repoDir, args, options = {}) {
  return execFileSync("git", args, {
    cwd: repoDir,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
    ...options,
  }).trim();
}

export function runGitInherit(repoDir, args) {
  execFileSync("git", args, { cwd: repoDir, stdio: "inherit" });
}

export function localBranchExists(repoDir, branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`], {
      cwd: repoDir,
      stdio: "ignore",
    });
    return true;
  } catch {
    return false;
  }
}

export function refExists(repoDir, refName) {
  try {
    execFileSync("git", ["rev-parse", "--verify", "--quiet", refName], {
      cwd: repoDir,
      stdio: "ignore",
    });
    return true;
  } catch {
    return false;
  }
}

export function gitCheckoutExists(absDir) {
  return fs.existsSync(path.join(absDir, ".git"));
}

export function currentBranchInRepo(repoDir) {
  return runGitInRepo(repoDir, ["branch", "--show-current"]);
}

export function headShaInRepo(repoDir) {
  return runGitInRepo(repoDir, ["rev-parse", "HEAD"]);
}

export function dirtyInRepo(repoDir) {
  return runGitInRepo(repoDir, ["status", "--porcelain=v1"]).length > 0;
}

export function listLocalBranches(repoDir) {
  const out = runGitInRepo(repoDir, ["for-each-ref", "--format=%(refname:short)", "refs/heads"]);
  return out.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).sort(compareStrings);
}

export function listRemoteHeads(repoDir = REPO_ROOT) {
  const out = runGitInRepo(repoDir, ["ls-remote", "--heads", "origin"]);
  const rows = out.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).map((line) => {
    const [sha, ref] = line.split(/\s+/);
    const branch = String(ref || "").replace("refs/heads/", "");
    return { branch, sha };
  });
  rows.sort((a, b) => compareStrings(a.branch, b.branch));
  return rows;
}

export function discoverGitCheckouts() {
  const entries = fs.readdirSync(WORKSPACE_ROOT, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => {
      const absDir = path.join(WORKSPACE_ROOT, entry.name);
      return {
        id: entry.name,
        abs_dir: absDir,
        rel_path: relFromRepo(absDir),
        is_git_checkout: gitCheckoutExists(absDir),
      };
    })
    .filter((entry) => entry.is_git_checkout)
    .sort((a, b) => compareStrings(a.id, b.id));
  return entries;
}

export function buildTopologyRegistry() {
  return {
    schema_version: SCHEMA_VERSION,
    canonical_branch: "main",
    protected_local_branches: [...PROTECTED_BRANCHES],
    protected_remote_branches: PROTECTED_BRANCHES.map((branch) => `origin/${branch}`),
    protected_worktrees: WORKTREE_SPECS.map((spec) => ({
      id: spec.id,
      rel_path: spec.rel_path,
      role: spec.role,
      canonical: spec.canonical,
      local_branch: spec.local_branch,
      remote_branch: spec.remote_branch,
      description: spec.description,
    })),
    helper_commands: {
      backup_snapshot: "just backup-snapshot",
      backup_status: "just backup-status",
      sync_all_role_worktrees: "just sync-all-role-worktrees",
      reseed_permanent_worktree_from_main: "just reseed-permanent-worktree-from-main",
      enumerate_cleanup_targets: "just enumerate-cleanup-targets",
      delete_local_worktree: "just delete-local-worktree",
      ensure_permanent_backup_branches: "just ensure-permanent-backup-branches",
    },
    backup_policy: {
      backup_push_before_destructive_local_git: true,
      immutable_snapshot_before_topology_deletion: true,
      nas_copy_mode: "timestamped_copy_no_mirror_deletes",
      backup_root_env_var: "HANDSHAKE_BACKUP_ROOT",
      nas_backup_root_env_var: "HANDSHAKE_NAS_BACKUP_ROOT",
    },
  };
}

export function renderTopologyRegistryMd(registry) {
  const lines = [
    "# GIT_TOPOLOGY_REGISTRY",
    "",
    "This file is a deterministic governance registry for the permanent Handshake checkout topology.",
    "",
    `- SCHEMA_VERSION: ${registry.schema_version}`,
    `- CANONICAL_BRANCH: ${registry.canonical_branch}`,
    `- PROTECTED_LOCAL_BRANCHES: ${registry.protected_local_branches.join(", ")}`,
    `- PROTECTED_REMOTE_BRANCHES: ${registry.protected_remote_branches.join(", ")}`,
    "",
    "## PROTECTED_WORKTREES",
    "",
    "| ID | ROLE | REL_PATH | LOCAL_BRANCH | REMOTE_BRANCH | CANONICAL | DESCRIPTION |",
    "| --- | --- | --- | --- | --- | --- | --- |",
  ];

  for (const row of registry.protected_worktrees) {
    lines.push(`| ${row.id} | ${row.role} | ${row.rel_path} | ${row.local_branch} | ${row.remote_branch} | ${row.canonical ? "YES" : "NO"} | ${row.description} |`);
  }

  lines.push(
    "",
    "## HELPER_COMMANDS",
    "",
    `- backup_snapshot: ${registry.helper_commands.backup_snapshot}`,
    `- backup_status: ${registry.helper_commands.backup_status}`,
    `- sync_all_role_worktrees: ${registry.helper_commands.sync_all_role_worktrees}`,
    `- reseed_permanent_worktree_from_main: ${registry.helper_commands.reseed_permanent_worktree_from_main}`,
    `- enumerate_cleanup_targets: ${registry.helper_commands.enumerate_cleanup_targets}`,
    `- delete_local_worktree: ${registry.helper_commands.delete_local_worktree}`,
    `- ensure_permanent_backup_branches: ${registry.helper_commands.ensure_permanent_backup_branches}`,
    "",
    "## BACKUP_POLICY",
    "",
    `- BACKUP_PUSH_BEFORE_DESTRUCTIVE_LOCAL_GIT: ${registry.backup_policy.backup_push_before_destructive_local_git ? "YES" : "NO"}`,
    `- IMMUTABLE_SNAPSHOT_BEFORE_TOPOLOGY_DELETION: ${registry.backup_policy.immutable_snapshot_before_topology_deletion ? "YES" : "NO"}`,
    `- NAS_COPY_MODE: ${registry.backup_policy.nas_copy_mode}`,
    `- BACKUP_ROOT_ENV_VAR: ${registry.backup_policy.backup_root_env_var}`,
    `- NAS_BACKUP_ROOT_ENV_VAR: ${registry.backup_policy.nas_backup_root_env_var}`,
    "",
  );
  return `${lines.join("\n")}\n`;
}

export function buildDynamicTopologySnapshot() {
  const registry = buildTopologyRegistry();
  const protectedWorktrees = registry.protected_worktrees.map((spec) => {
    const absDir = absFromRepo(spec.rel_path);
    const exists = fs.existsSync(absDir);
    const repoOk = exists && gitCheckoutExists(absDir);
    const snapshot = {
      id: spec.id,
      rel_path: spec.rel_path,
      role: spec.role,
      local_branch: spec.local_branch,
      remote_branch: spec.remote_branch,
      exists,
      repo_ok: repoOk,
    };
    if (repoOk) {
      snapshot.current_branch = currentBranchInRepo(absDir);
      snapshot.head_sha = headShaInRepo(absDir);
      snapshot.dirty = dirtyInRepo(absDir);
      snapshot.local_branches = listLocalBranches(absDir);
    }
    return snapshot;
  });

  const discoveredCheckouts = discoverGitCheckouts().map((entry) => {
    const snapshot = {
      id: entry.id,
      rel_path: entry.rel_path,
      current_branch: currentBranchInRepo(entry.abs_dir),
      head_sha: headShaInRepo(entry.abs_dir),
      dirty: dirtyInRepo(entry.abs_dir),
    };
    return snapshot;
  });

  return {
    schema_version: DYNAMIC_SNAPSHOT_SCHEMA_VERSION,
    canonical_branch: registry.canonical_branch,
    protected_remote_heads: listRemoteHeads(REPO_ROOT)
      .filter((row) => PROTECTED_BRANCHES.includes(row.branch)),
    protected_worktrees: protectedWorktrees,
    discovered_git_checkouts: discoveredCheckouts,
  };
}

export function ensureDir(absDir) {
  fs.mkdirSync(absDir, { recursive: true });
}

export function writeFileNormalized(absPath, contents) {
  ensureDir(path.dirname(absPath));
  fs.writeFileSync(absPath, contents, "utf8");
}

export function timestampForSnapshot(now = new Date()) {
  const yyyy = String(now.getUTCFullYear());
  const mm = String(now.getUTCMonth() + 1).padStart(2, "0");
  const dd = String(now.getUTCDate()).padStart(2, "0");
  const hh = String(now.getUTCHours()).padStart(2, "0");
  const mi = String(now.getUTCMinutes()).padStart(2, "0");
  const ss = String(now.getUTCSeconds()).padStart(2, "0");
  return `${yyyy}${mm}${dd}-${hh}${mi}${ss}Z`;
}

function readPersistedUserEnv(name) {
  if (process.platform !== "win32") return "";
  try {
    return execFileSync(
      "powershell.exe",
      ["-NoLogo", "-NonInteractive", "-Command", `[Environment]::GetEnvironmentVariable('${name}','User')`],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    ).trim();
  } catch {
    return "";
  }
}

export function resolveBackupRoot(overrideValue = "") {
  const value = String(overrideValue || process.env.HANDSHAKE_BACKUP_ROOT || readPersistedUserEnv("HANDSHAKE_BACKUP_ROOT") || "").trim();
  if (value) return path.resolve(value);
  return path.resolve(WORKSPACE_ROOT, "..", "Handshake Backups");
}

export function resolveNasBackupRoot(overrideValue = "") {
  const value = String(overrideValue || process.env.HANDSHAKE_NAS_BACKUP_ROOT || readPersistedUserEnv("HANDSHAKE_NAS_BACKUP_ROOT") || "").trim();
  return value ? path.resolve(value) : "";
}

export function runRobocopy(sourceDir, destDir, extraArgs = []) {
  const args = [
    sourceDir,
    destDir,
    "/E",
    "/COPY:DAT",
    "/R:1",
    "/W:1",
    "/XJ",
    "/NFL",
    "/NDL",
    "/NJH",
    "/NJS",
    "/NP",
    "/XD",
    ...SNAPSHOT_EXCLUDE_DIRS,
    "/XF",
    ...SNAPSHOT_EXCLUDE_FILES,
    ...extraArgs,
  ];
  const result = spawnSync("robocopy", args, { stdio: "inherit" });
  if (typeof result.status === "number" && result.status >= 8) {
    throw new Error(`robocopy failed with exit code ${result.status}`);
  }
}

export function parseOriginRepo() {
  const raw = runGit(["remote", "get-url", "origin"]);
  const normalized = raw.replace(/\.git$/, "");
  const httpsMatch = normalized.match(/^https:\/\/github\.com\/([^/]+)\/([^/]+)$/i);
  if (httpsMatch) return { owner: httpsMatch[1], repo: httpsMatch[2] };
  const sshMatch = normalized.match(/^git@github\.com:([^/]+)\/([^/]+)$/i);
  if (sshMatch) return { owner: sshMatch[1], repo: sshMatch[2] };
  return { owner: "", repo: "" };
}
