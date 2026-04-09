#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("ensure-permanent-backup-branches.mjs", { role: "SHARED" });

const PERMANENT_BRANCHES = ["main", "user_ilja", "gov_kernel"];

function runGit(args, options = {}) {
  return execFileSync("git", args, { stdio: "pipe", encoding: "utf8", ...options }).trim();
}

function runGitInherit(args) {
  execFileSync("git", args, { stdio: "inherit" });
}

function fail(message, details = []) {
  failWithMemory("ensure-permanent-backup-branches.mjs", message, { role: "SHARED", details });
}

function localBranchExists(branch) {
  try {
    execFileSync("git", ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

function remoteBranchExists(remoteName, branch) {
  const out = runGit(["ls-remote", "--heads", remoteName, branch]);
  return out.length > 0;
}

function main() {
  if (!localBranchExists("main")) {
    fail("Local main branch not found", ["Cannot seed permanent backup branches without refs/heads/main"]);
  }

  for (const branch of PERMANENT_BRANCHES) {
    if (remoteBranchExists("origin", branch)) {
      console.log(`[ENSURE_PERMANENT_BACKUP_BRANCHES] exists: origin/${branch}`);
      continue;
    }
    console.log(`[ENSURE_PERMANENT_BACKUP_BRANCHES] creating origin/${branch} from local main`);
    runGitInherit(["push", "origin", `refs/heads/main:refs/heads/${branch}`]);
  }

  console.log("[ENSURE_PERMANENT_BACKUP_BRANCHES] done");
}

main();
