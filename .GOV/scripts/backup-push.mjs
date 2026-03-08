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
  fail("Usage: node .GOV/scripts/backup-push.mjs [local_branch] [remote_branch]", [
    "Example (current branch -> same remote branch): node .GOV/scripts/backup-push.mjs",
    "Example (explicit): node .GOV/scripts/backup-push.mjs role_orchestrator role_orchestrator",
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

function workingTreeDirty() {
  return runGit(["status", "--porcelain=v1"]).length > 0;
}

function main() {
  const localBranch = (process.argv[2] || "").trim() || currentBranch();
  const remoteBranch = (process.argv[3] || "").trim() || localBranch;

  if (!localBranch) usage();
  if (!branchExists(localBranch)) {
    fail("Local branch not found", [`branch=${localBranch}`]);
  }

  if (workingTreeDirty()) {
    fail("Working tree is dirty; backup push only captures committed state.", [
      "Commit the intended snapshot first.",
      `Then run: just backup-push ${localBranch} ${remoteBranch}`,
    ]);
  }

  runGitInherit(["push", "-u", "origin", `${localBranch}:${remoteBranch}`]);
  console.log(`[BACKUP_PUSH] origin/${remoteBranch} now tracks ${localBranch}`);
}

main();
