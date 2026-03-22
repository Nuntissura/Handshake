#!/usr/bin/env node

import { execFileSync } from "node:child_process";

function runGit(args, options = {}) {
  return execFileSync("git", args, { stdio: "pipe", encoding: "utf8", ...options }).trim();
}

function runGitInherit(args) {
  execFileSync("git", args, { stdio: "inherit" });
}

function fail(message, details = []) {
  console.error(`[BACKUP_PUSH] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function usage() {
  fail("Usage: node .GOV/roles_shared/scripts/topology/backup-push.mjs [local_branch] [remote_branch]", [
    "Example (current branch -> same remote branch): node .GOV/roles_shared/scripts/topology/backup-push.mjs",
    "Example (explicit): node .GOV/roles_shared/scripts/topology/backup-push.mjs gov_kernel gov_kernel",
  ]);
}

function branchExists(branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

function currentBranch() {
  return runGit(["branch", "--show-current"]);
}

function relevantDirtyPaths() {
  const output = runGit(["status", "--porcelain=v1", "--untracked-files=all"]);
  if (!output) return [];
  const isGovJunctionNoise = (path) => /^\.?GOV\//.test(path);
  return output
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .map((line) => line.slice(3).trim())
    .map((rawPath) => {
      const arrowIndex = rawPath.indexOf(" -> ");
      const normalized = (arrowIndex >= 0 ? rawPath.slice(arrowIndex + 4) : rawPath)
        .replace(/^"(.*)"$/, "$1")
        .replace(/\\/g, "/");
      return normalized;
    })
    .filter((path) => path && !isGovJunctionNoise(path));
}

function workingTreeDirty() {
  return relevantDirtyPaths().length > 0;
}

function main() {
  const localBranch = (process.argv[2] || "").trim() || currentBranch();
  const remoteBranch = (process.argv[3] || "").trim() || localBranch;

  if (!localBranch) usage();
  if (!branchExists(localBranch)) {
    fail("Local branch not found", [`branch=${localBranch}`]);
  }

  if (workingTreeDirty()) {
    const dirtyPaths = relevantDirtyPaths();
    fail("Working tree is dirty; backup push only captures committed state.", [
      `Relevant dirty paths after .GOV filter: ${dirtyPaths.length ? dirtyPaths.join(", ") : "<none>"}`,
      "Commit the intended snapshot first.",
      "Ignored dirt filter: .GOV/ junction churn is excluded for WP feature-branch safety pushes.",
      `Then run: just backup-push ${localBranch} ${remoteBranch}`,
    ]);
  }

  runGitInherit(["push", "-u", "origin", `${localBranch}:${remoteBranch}`]);
  console.log(`[BACKUP_PUSH] origin/${remoteBranch} now tracks ${localBranch}`);
}

main();
