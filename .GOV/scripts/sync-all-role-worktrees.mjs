#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import {
  WORKTREE_SPECS,
  absFromRepo,
  currentBranchInRepo,
  dirtyInRepo,
  gitCheckoutExists,
  localBranchExists,
  runGitInherit,
} from "./git-topology-lib.mjs";

for (const spec of WORKTREE_SPECS) {
  const absDir = absFromRepo(spec.rel_path);
  if (!fs.existsSync(absDir) || !gitCheckoutExists(absDir)) {
    console.log(`[SYNC_ALL_ROLE_WORKTREES] skip missing checkout: ${spec.id} (${spec.rel_path})`);
    continue;
  }

  if (dirtyInRepo(absDir)) {
    throw new Error(`Refusing to sync dirty checkout: ${spec.id} (${spec.rel_path})`);
  }

  const originalBranch = currentBranchInRepo(absDir);
  runGitInherit(absDir, ["fetch", "origin"]);

  if (localBranchExists(absDir, "main")) {
    runGitInherit(absDir, ["checkout", "main"]);
    runGitInherit(absDir, ["merge", "--ff-only", "origin/main"]);
  }

  if (localBranchExists(absDir, spec.local_branch)) {
    runGitInherit(absDir, ["checkout", spec.local_branch]);
    runGitInherit(absDir, ["merge", "--ff-only", spec.remote_branch]);
  }

  if (localBranchExists(absDir, originalBranch)) {
    runGitInherit(absDir, ["checkout", originalBranch]);
  }

  console.log(`[SYNC_ALL_ROLE_WORKTREES] ok: ${spec.id}`);
}
